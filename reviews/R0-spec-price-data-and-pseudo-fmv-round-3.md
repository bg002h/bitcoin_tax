# R0 — SPEC_price_data_and_pseudo_fmv — round 3

**Artifact:** `design/SPEC_price_data_and_pseudo_fmv.md` (round-2 folded IN-PLACE — I-A/I-B/M-A/M-B/M-C).
**Baseline reviewed against:** branch `feat/price-data-fmv` @ `8d2ef1a` (spec commit "spec(price-fmv): fold R0
round 2"); source verified against this tree (main == `019ed3f`). Read-only architect review; no implementation
performed.
**Prior rounds:** `reviews/R0-spec-price-data-and-pseudo-fmv-round-{1,2}.md` (r1 BLOCKED 1C/4I/5M/3N;
r2 BLOCKED 0C/2I/3M/2N).
**Bar:** 0 Critical / 0 Important.

## Verdict: **R0-GREEN — 0 Critical / 0 Important / 3 Minor / 2 Nit**

Both round-2 Importants (I-A test-blast-radius + Session seam; I-B income taint enumeration) and all three
Minors (M-A dirs location, M-B cargo-tree mechanism, M-C transitive-dep wording) are **folded correctly and
verified against current source** — every load-bearing citation re-checked holds (see the fold-verification
table). The final cross-crate completeness sweep — the class of miss that recurred in rounds 1 and 2 — turned up
**one** genuinely un-enumerated construction-site break (`btctax-adapters/tests/kat_rate_engine.rs`) plus a
seam-plumbing precision note, but neither rises to Important: the affected crate is already in the
Scope/lockstep list and version-bumped, the fix is 100% compiler-forced and mechanical (no stub-value
recomputation, no design decision), and the "full suite green" phase gate is a hard catch. The three seams
(A = provider injection, B = resolve-layer synthetic FMV + taint, C = separate `ureq` crate) are sound; the
Plan (T1/T2/T3) is implementable with no open blocking question; one combined spec remains defensible.

Cleared to implement.

---

## Fold verification (round-2 findings re-checked against `8d2ef1a` source)

| R2 | Claim to verify | Verified? | Evidence (file:line) |
|----|-----------------|-----------|----------------------|
| **I-A test** | tui-edit `seed_income_inbounds` KAT breaks in BOTH modes (exact-FMV pins $84.00/$33.75/$117.75 + the 2025-04-01 sentinel) | **YES** | `btctax-tui-edit/src/main.rs:21065-21067` seeds i1=2025-03-01/100k, i2=2025-06-15/50k, i3=2025-04-01/40k(UNPRICED); `:21260` `m.count==2`, `:21263` `total_income_usd==dec!(117.75)`, `:21264` "$84.00 (i1) + $33.75 (i2)", `:21267` `excluded_missing_price==1`. The stub CSV (`data/btc_usd_daily_close.csv`, 6 data rows) has 2025-03-01→84000 (100k sat=$84.00), 2025-06-15→67500 (50k sat=$33.75), and **no** 2025-04-01 → i3 excluded. Real data changes both closes AND covers 2025-04-01 ⇒ breaks both modes. Spec lines 47-50 enumerate this exactly. |
| **I-A route** | routes through `Session::bulk_classify_income_plan` (session.rs:771/776) | **YES** | `session.rs:771` `pub fn bulk_classify_income_plan`, `:776` `let prices = BundledPrices::load()?;`. The KAT drives it via `handle_key`/`bulk_income_modal` (`main.rs:21255-21259`), not a free `project()` — so it needs an instance-level provider, per spec line 51-58. |
| **I-A seam** | core `project()` already takes `&dyn PriceProvider`; the seam is a cli-layer refactor | **YES** | `btctax-core/src/project/mod.rs:62-64` `pub fn project(events, prices: &dyn PriceProvider, config)`. Hard-wire is only at the cli Session/cmd layer. Spec lines 51-54 state this. |
| **I-A enum** | is the `BundledPrices::load()` + stub-value enumeration COMPLETE across ALL crates? | **YES (1 Minor)** | Re-grepped: cli = 17 load sites (`session.rs` ×10: 449/463/479/514/554/589/678/776/886/1046; `cmd/reconcile.rs` ×4: 240/271/478/552; `cmd/optimize.rs` ×3: 44/111/179) — spec's "~15 in the cli" is approximate. Also `adapters/src/ingest.rs:29` + `tui-edit/src/main.rs:6526` (live). Stub values (84000/67500/42500/84250/6.75/84.00/33.75/117.75) all trace to the enumerated tests. **Minor M-D** on the 7 cmd-layer free-fn sites. |
| **I-B pushes** | BOTH push sites `fold.rs:689` (native) + `fold.rs:877` (IncomeInbound) get the field | **YES** | `fold.rs:689` (`Op::Income` arm) + `fold.rs:877` (`Op::IncomeInbound` arm) both `st.income_recognized.push(IncomeRecord { event, recognized_at, sat, usd_fmv, kind, business })` — identical field lists, no `pseudo`. `state.rs:211-218` struct has no `pseudo`. Spec line 74-75 names both. |
| **I-B helper** | `pseudo_tag` (render.rs:61) is the real per-row helper; `pseudo_marker` is not | **YES** | `render.rs:61` `fn pseudo_tag(pseudo: bool) -> &'static str` (used at Lot `:239`, legs `:353/:365` per r2). Grep for `pseudo_marker` across `crates/` finds only a **test fn name** (`pseudo_reconcile_cli.rs:81`) — no such render helper exists. Spec lines 76-77 name `pseudo_tag`; the parenthetical "the unused `pseudo_marker`" is a harmless dangling reference (Nit N-A). |
| **I-B tui** | `btctax-tui` has ~11 `IncomeRecord` construction sites needing the field to compile | **YES (exact)** | `rg -c "IncomeRecord \{"`: `tabs/tests.rs`=7, `tabs/income.rs`=2, `export.rs`=1, `lib.rs`=1 ⇒ **exactly 11**. Spec line 82 "~11 fixture sites" is precise. `btctax-tui` + `-tui-edit` are now BOTH in the Scope/lockstep list (spec lines 132-133). |
| **I-B tui-edit** | tui-edit needs the field threaded through construction | **YES (clarified)** | `rg "IncomeRecord \{"` in tui-edit = **0 construction sites** — tui-edit only *consumes* projected `IncomeRecord`s (the "+ the projection" in spec line 82). So it **recompiles**, no constructor edits — consistent with the r2 review's "plus recompiles in btctax-tui-edit." |
| **I-B TUI guard** | TUI-surfaces-via-banner (draw_edit.rs) decision + per-row-marker = FOLLOWUP is sound | **YES** | `draw_edit.rs` pseudo banner: "PSEUDO-RECONCILE MODE ACTIVE … [PSEUDO] rows are FICTIONAL placeholders — DO NOT FILE. Export blocked." (the `if show_banner` block). Spec lines 78-83 adopt the banner+field for TUI income and route a per-row TUI marker to FOLLOWUP. Sound — matches the existing Lot/DisposalLeg banner convention. |
| **M-A** | `dirs` in cli + update-prices, NOT adapters; no residual contradiction | **YES** | Re-scanned every `dirs` mention: lines 100-103 (cli+updater resolve path; adapters takes `cache_path: Option<&Path>`, "no `dirs`, no network"), 116 (update-prices: adapters+`dirs`+ureq), 129 (adapters "**no `dirs`**"), 131 (cli "`dirs` cache-path"), 133 (update-prices "ureq + `dirs`"), 145 (T3 "`dirs`, no ureq"). **All consistent — the r2 line-87/111 contradiction is gone.** |
| **M-B** | cargo-tree isolation = an xtask/CI step, not a `#[test]` | **YES** | Spec line 120 "an **xtask/CI step** (NOT a non-hermetic `#[test]`) asserting `ureq`/rustls is absent from `cargo tree`". Fixed. |
| **M-C** | core is transitive-via-adapters; no DIRECT core/cli dep | **YES** | Spec lines 116-117 "no **DIRECT** dep on btctax-core/cli … (btctax-core arrives TRANSITIVELY via adapters — fine: core is itself network-free)." Correct. |
| **M3** | versions 0.2.0→0.3.0; 8→9 members | **YES** | All 8 `crates/*/Cargo.toml` at `version = "0.2.0"`. `Cargo.toml` members = 8 (`btctax, -store, -core, -adapters, -cli, -tui, -tui-edit, xtask`) = 7 publishable + xtask; +update-prices ⇒ 9 = 8 publishable + xtask. No HTTP client (`ureq/reqwest/hyper/tokio/rustls`) anywhere today. Spec lines 134-135 correct. |
| **B mechanism** | native income has no price fallback (resolve.rs) — the pseudo `manual_fmv` injection is the right seam | **YES** | `resolve.rs:287-306` Income arm: `fmv = fmv_override.or_else(|| x.usd_fmv.filter(|_| x.fmv_status != Missing))` — **no `fmv_of` fallback**. A pseudo synthetic injected into the `manual_fmv` map is picked up by `fmv_override`, flowing to `Op::Income` → the `fold.rs:689` push. Spec lines 70-72 correct. |

---

## MINOR

### M-D — `btctax-adapters/tests/kat_rate_engine.rs` constructs `IncomeRecord` (2 sites) — the last un-enumerated cross-crate field-break

The final sweep the prompt asked for ("any OTHER `IncomeRecord {` construction site the field addition breaks")
turns up **one** the spec's "so they COMPILE" enumeration (line 82, which names only `btctax-tui` +
`-tui-edit`) does not mention:

- `crates/btctax-adapters/tests/kat_rate_engine.rs:167` (`state_with_mining`) and `:190` (`state_lt_with_mining`)
  construct `IncomeRecord { event, recognized_at, sat, usd_fmv, kind, business }` with **no** `..Default`. Adding
  a non-`Default` `pub pseudo: bool` (spec line 74) is compiler-forced at both.

**Why this is Minor, not a third recurrence of the r1/r2 Important class:** unlike the r1/r2 misses (an entire
crate ABSENT from Scope/lockstep, hiding a *design* decision — the seam reach, the TUI marker policy),
`btctax-adapters` is **already in the Scope/SemVer/lockstep list** (spec line 129) and version-bumped, so it is
compiled + tested + shipped in the delivery. And these two sites take `usd_fmv` as a **passed-in parameter**
(`amount`/`mining`, `kat_rate_engine.rs:165/181`) — **not** resolved from the bundled dataset — so they do NOT
break on the A data-swap and require **no** value recomputation; the fix is a purely mechanical `pseudo: false`
the compiler points straight at, caught by T2's "full suite green" gate. No design or data decision is deferred.

**Suggested fold (optional, non-blocking):** append to spec line 82 "(+ the 2 `btctax-adapters`
`kat_rate_engine.rs` fixtures and `btctax-core`'s own income test helpers — mechanical `pseudo: false`)."

### M-E — the "Session-level provider" label is narrower than the full ~15 cli `load()` set

Spec lines 51-54 call the seam "an instance-level provider on `Session`" covering "~15 `BundledPrices::load()`
sites." But **7** of the cli sites are cmd-layer **free functions**, not `Session` methods:
`cmd/reconcile.rs:240/271/478/552` + `cmd/optimize.rs:44/111/179`. A provider stored on `Session` does not
automatically reach a free `cmd::reconcile` fn that calls `BundledPrices::load()` itself — and the
`cli/tests/reconcile.rs` `$84.00/$33.75/$27.00` pins (~48 refs, spec line 42) drive exactly that cmd path.

This is Minor (not Important) because the spec already supplies a **complete** set of migration mechanisms for
these tests: inject the synthetic provider (where the cmd fn can take it), else "recompute expected values only
where a test genuinely asserts bundled coverage" (line 57), else the ">2026-06-03 unpriced far-future fallback …
mark refresh-fragile" (line 58). Every stub-coupled test therefore has a defined path and the T1 gate forces
correctness — no open blocking question. Recommend tightening the label to "an injectable provider threaded
through the cli price-loading layer (`Session` methods **and** the `cmd/reconcile.rs` + `cmd/optimize.rs` free
fns)" so an implementer does not assume a `Session` field alone suffices.

### M-F — stale post-swap comments (carried from r2 N-A, still open)

`optimize_consult.rs:408` ("The bundled dataset's last entry is 2025-06-15") and `reclassify_income_cli.rs:22`
(`$84,000`) go stale once the real 5,802-row dataset lands. Refresh during the T1 migration. (Non-blocking;
same class as r1 N2 / r2 N-A.)

---

## NIT

- **N-A — dangling `pseudo_marker` reference.** Spec lines 76-77 say use `pseudo_tag` "NOT the unused
  `pseudo_marker`." Verified there is **no** `fn pseudo_marker` in the tree (the only hit is the test-fn name
  `pseudo_marker_on_screen_but_absent_from_every_export_file`, `pseudo_reconcile_cli.rs:81`). The actionable
  instruction (`pseudo_tag` at `render.rs:61`) is correct; the "NOT `pseudo_marker`" aside now references a
  non-existent symbol — drop it or it will confuse an implementer grepping for it.
- **N-B — `tui-edit/src/main.rs:6526` live `BundledPrices::load()` is outside the test seam.** The
  `handle_pseudo_approve_modal_key` path loads bundled prices directly (a LIVE, non-test path). Correct as-is for
  T1 (no test pins its output against the stub, so the seam need not reach it). Flagging only so T3's
  `LayeredPrices`/cache wiring remembers to give this live path the cache-aware provider too (`load_with_cache`),
  same as the Session sites — a T3 implementation detail, already implied by the provider-level cache design.

---

## Confirmations (verified sound — no change needed)

- **The C1 test-migration enumeration is now materially complete.** Three crates named
  (`btctax-adapters`, `btctax-cli`, `btctax-tui-edit`), both failure modes (exact-FMV pins + no-price
  sentinels), the seam feasibility (core already `&dyn PriceProvider`), and three fallback mechanisms
  (inject / recompute / far-future). The only residual is the mechanical `kat_rate_engine.rs` field-add (M-D).
- **The income-taint guard is closed end-to-end.** Field at both `fold.rs` pushes → `pseudo_tag` on the CLI
  report (the primary tax-output surface) → banner on the TUI → export writers OMIT it (the ★ headline guard;
  `render.rs:56-60` doc-comment mandates writers never call `pseudo_tag`). Per-row TUI marker correctly deferred
  to FOLLOWUP (no pre-existing per-row TUI convention to retrofit under #41's scope).
- **Part C network isolation remains clean.** Zero HTTP client in the workspace today; a `btctax-update-prices`
  binary depending on `btctax-adapters` + `dirs` + `ureq` only, with nothing depending back on it, keeps
  `ureq`/rustls out of every tax binary's `cargo tree` — asserted by an xtask/CI step (M-B), the right
  hermetic mechanism.
- **The Plan is implementable with 0 open blocking questions.** T1 (data + NOTICE + provider-injection seam +
  test migration + vault-income fixture [M5]) → T2 (`IncomeRecord.pseudo` + resolve-layer synthetic FMV + taint
  + render + `PseudoKind` + approve→`ManualFmv` + the ★ fault-inject) → T3 (`LayeredPrices` + cache + the new
  crate + dep-tree check + docs). Each phase carries its own KATs and stop-at-green.
- **One combined spec is still acceptable for A+B+C** (r2 N-B, re-affirmed): the parts are genuinely coupled
  (B's honesty depends on A's data via the "no price ⇒ stay blocked" fault-inject; C extends A's provider), and
  the phased T1/T2/T3 plan with per-phase gates + A's migration on its own commit is an adequate mitigation. Not
  required to split.
- **No residual internal contradiction** after three edit passes: `dirs` location consistent (M-A closed),
  member/version arithmetic correct, the I1 reversal of `SPEC_pseudo_reconcile_mode.md` (20/107) stated and the
  "0 blockers" contract amended (spec lines 66-69), the cache-as-local-input framing (I3) intact.

---

### Result
Round-2 folds confirmed correct against source; the cross-crate enumeration is now complete to the level of one
mechanical field-add (M-D) and a label-precision note (M-E), neither blocking. **0 Critical / 0 Important /
3 Minor / 2 Nit → R0-GREEN — cleared to implement.** Fold M-D/M-E/M-F/N-A into the spec opportunistically during
T1/T2 (they need no re-review gate); N-B is a T3 reminder.
