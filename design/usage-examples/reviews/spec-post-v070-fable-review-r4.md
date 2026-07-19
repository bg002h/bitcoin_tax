# Independent spec re-review — SPEC_post_v070_product_cycle.md (r4, FINAL)

**Reviewer:** Fable (independent; reviewed r1–r3; did not author the spec or any fold).
**Date:** 2026-07-18. **Branch:** `feat/post-v070-product-cycle`.
**Artifact:** `design/usage-examples/SPEC_post_v070_product_cycle.md` @ r4 (`752c20c`).
**Scope:** focused verification of the r3→r4 fold (`[R3-*]` tags; diff `d49dea4..752c20c` read in full)
plus a sweep for fold-introduced defects. Source re-verified at every anchor the fold leans on:
core `project/resolve.rs` (exhaustive `applied.insert` grep; conflict resolution :508-513, accept-first
:521-522, pass 1c :543-560 incl. `applied.contains_key` :551 and insert :558, pass 1d reads :576-578,
pass 1e reads :728-730/:789-791, pseudo Phase A :934-949, void revocability :423-440),
`cmd/reconcile.rs` (fn map re-pinned: :41/:62/:85/:110/:301/:1136; `pseudo_set_mode` :168),
`cli.rs` (`TaxProfile` :237-300 — `year` is `#[arg(long)]`; `Pseudo::On/Off/Approve` :846-877;
`IncomeCmd::Import` :358-360), cli `resolve.rs` ladder :100-130, core `state.rs`
(`pseudo_synthetic_count` :277, `pseudo_active()` :282).

---

## Part 1 — Resolution of the r3 findings

| r3 ID | Verdict | Basis |
|-------|---------|-------|
| **R3-I1** (effective view one channel short of `applied`) | **RESOLVED** | §3.2 now pins the view **definitionally** to the resolver's own `applied` under pseudo-OFF ("Do NOT hand-rebuild a subset … reuse the resolver's … construction in a shadow projection with `pseudo_reconcile` forced off, so the view is *definitionally* whatever the resolver sees"), and the writer enumeration is demoted to explanatory. **Fourth-writer audit (exhaustive):** `applied.insert` appears at exactly four sites — `:513` (accepted `SupersedeImport`, real), `:522` (accept-first, gated `pseudo_on`), `:558` (pass 1c `ClassifyRaw`, real, void-folded), `:949` (Phase A placeholder, inside `if pseudo_on`). Under pseudo-OFF the real writers are **exactly the two the spec names** — no missed channel remains; the view (applied overrides + `unwrap_or(&raw.payload)` fallback) is extensionally the resolver's. The `:551` duplicate-check fact (pass 1c keys on `applied.contains_key`, which includes the `:513` entries) re-verified in source — classify-raw on an accept-governed target IS a NEW `DecisionConflict`. **KAT covers both directions:** accept `set-fmv`/`reclassify-income` on an accept-governed Income target (reds a ClassifyRaw-only enumerated view, which false-refuses it); refuse `classify-raw` on an accept-governed target (reds a decisions-only duplicate check) plus the wrong-type accept-governed refuse case (reds a raw-log-only type check). All constructible (re-import conflict + `accept-conflict` are first-class). Two wording residuals, both Nits (R4-N1, R4-N2) — neither opens a divergence channel. |
| **R3-M1** (channel precedence / `count` two meanings) | **RESOLVED (decision-bearing half); one leg residual → R4-M2** | The new `[R3-min]` bullet makes the channels **definitionally disjoint**: synthetic requires `pseudo_synthetic_count > 0`, placeholder requires `count == 0` — the parallel construction pins `count` = `pseudo_synthetic_count` (verified `state.rs:277/:282`: `pseudo_active()` ⇔ count > 0). The underlying **states** still overlap (verified: the ladder's arm 3, cli `resolve.rs:120-128`, injects on bare `if pseudo_reconcile` with **no count condition** — so `PseudoPlaceholder` provenance + count > 0 co-occurs), but in that overlap only the synthetic **channel** fires, and r3 already established that banner is true clause-by-clause there. "No precedence rule is needed" is therefore correct — the `count == 0` conjunct *is* the precedence rule. Residual: the fix's third leg (tighten the inject-description gloss) was not folded — R4-M2. |
| **R3-M2** (nonexistent `--set` flag in the placeholder banner) | **NOT RESOLVED — replaced with a different dead spelling → R4-M1** | The fold swapped `--set` for "`btctax tax-profile <year> …`", but `TaxProfile` has **no positional year**: `year: i32` is `#[arg(long)]` (`cli.rs:238-240`), so `btctax tax-profile 2025 …` is clap-rejected exactly like `--set` was. r3's prescribed spelling was `--year <Y>`; the fold dropped the `--`. The other two new pointers ARE live (`btctax income import` — `IncomeCmd::Import`, `cli.rs:358-360`; `btctax reconcile pseudo off` — `Pseudo::Off`, `cli.rs:853-855`), and "setting is the default; `--show` inverts" is accurate. Same severity as r3 gave it (self-correcting via clap's usage error; no figure/refusal impact) — Minor, but it is now the **second consecutive dead spelling** on the novice-facing banner, and KAT (b) will bake whatever is pinned into the shipped binary. Fix before the PLAN freezes the text. |
| **R3-M3** (choke list omits `classify_raw`) | **RESOLVED** | Choke list now `reconcile.rs:41/62/85/110/301/1136` — all six re-pinned exact this pass (`classify_inbound` :41, `reclassify_outflow` :62, `set_fmv` :85, `void` :110, **`classify_raw` :301**, `reclassify_income` :1136), and the KAT names a `classify-raw` refuse case, so a PLAN that leaves `classify_raw` unvalidated reds. |
| **R3-N1** (`--sell -1` KAT non-red) | **RESOLVED** | §3.3 KAT now spells **`--sell=-1`** with a message assert and records *why* the space form cannot witness the guard (clap-rejected pre-fix) — the untested-guard trap is closed and documented against reintroduction. |

## Part 2 — Fold-introduced-defect sweep

- **KAT voided-variant antecedent (new in r4):** the accept list now ends "…from a live real
  `ClassifyRaw` or from an accepted `SupersedeImport` conflict; the same target with **that decision**
  voided → refused wrong-type." Read distributively, the `SupersedeImport` arm is **unbuildable**:
  `SupersedeImport` is non-revocable (verified `resolve.rs:423-440`; §3.2's own void bullet refuses it),
  so the void step itself is refused and "refused wrong-type" is unreachable. r3's fix text said
  explicitly "no voided-variant case exists here"; the fold dropped that scoping. Self-detecting (the
  KAT author hits the loud void refusal immediately) and both escape readings land on correct behavior,
  but it is a literal internal contradiction with the void bullet — **R4-M3**.
- **"Three real writers" label:** the real `applied` writers are **two** (`:513`, `:558`); the third
  item ("(for existence) the raw event log") is the `unwrap_or` *fallback*, not a writer, and the
  actual 3rd/4th writers (`:522`, `:949`) are pseudo-gated and correctly excluded by the pseudo-OFF
  mandate. Extensionally the described view is exactly the resolver's, so no divergence channel opens —
  wording only, **R4-N1**.
- **"Reuse the resolver's pass-1c/1d/1e construction":** the `:513` writer lives in the
  conflict-resolution pass (`:508-513`), *before* 1c — the span label under-names it. Harmless in
  context (the same paragraph names `:513` explicitly, the shadow projection runs the whole resolve,
  and the accept-governed KAT cases red any construction that misses it) — **R4-N2**.
- **Cross-section consistency:** §8's summary line ("effective-payload pseudo-OFF view") matches r4
  §3.2; §3.5's clause-4a/4b cross-refs and §3.1's four surfaces are untouched by the fold and remain
  consistent (r3-verified). §9 anchors untouched; every NEW inline cite the fold added (`:513`, `:551`,
  `:543-560`, `:423-440`, `reconcile.rs:301`) verified exact against current source this pass.
- **KAT red-capability:** all r4-added cases re-checked red-capable (Part 1, R3-I1 row); the
  `--sell=-1` respelling restores mutation-redness; no other KAT weakened by the fold.
- No new contradiction found across §3.1/§3.2/§3.5.

## Part 3 — Findings (all non-blocking)

- **R4-M1 (Minor, carry-over of R3-M2):** placeholder-banner remedy still names a dead spelling —
  `btctax tax-profile <year> …` (positional) where the CLI takes `--year <Y>` (`cli.rs:238-240`).
  One-token fix: "`btctax tax-profile --year <Y> …`". Must be folded before KAT (b) pins the text.
- **R4-M2 (Minor, residual leg of R3-M1):** §3.1's inject gloss still reads "when `cfg.pseudo_reconcile`
  is on and nothing is stored (`count == 0`)" — the inject (`resolve.rs:120-128`) has **no** count
  condition, and with `count` now pinned to `pseudo_synthetic_count` the parenthetical is a false claim
  about source (harmless: the channel definitions are self-contained, no reading yields a false banner).
  Fix: delete the parenthetical or gloss as "no stored profile/inputs".
- **R4-M3 (Minor, new):** scope the §3.2 KAT's voided-variant to the `ClassifyRaw` case — add
  "(ClassifyRaw case only — `SupersedeImport` is non-revocable, `resolve.rs:423-440`)".
- **R4-N1 (Nit):** "three real writers" → "two real writers (`:513`, `:558`) plus the raw-payload
  fallback for existence".
- **R4-N2 (Nit):** "pass-1c/1d/1e construction" → "conflict-resolution + pass-1c/1d/1e construction"
  (or simply "the resolver's `applied` construction").

## Part 4 — Verdict

R3-I1 is genuinely resolved: the view is now pinned definitionally to the resolver's pseudo-OFF
`applied` (exhaustive writer audit confirms no remaining real channel), and the KAT exercises the
accept-governed accept AND refuse directions, both red-capable. R3-M1 (decision-bearing half), R3-M3,
and R3-N1 are folded and anchor-exact. The residue is three one-token/one-clause spec-text Minors
(a still-dead CLI spelling in the placeholder banner, a stale inject gloss, an over-broad KAT
antecedent) and two wording Nits — none opens a correctness, divergence, or false-refusal channel,
and none holds the gate. Fold them inline (a cosmetic r5) or burn them down at PLAN time before
KAT (b) freezes the banner text; R4-M1 in particular must not reach the shipped binary.

**VERDICT: 0 Critical / 0 Important / 3 Minor (R4-M1, R4-M2, R4-M3) / 2 Nit (R4-N1, R4-N2) — GREEN.**
