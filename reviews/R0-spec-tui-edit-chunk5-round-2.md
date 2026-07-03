# R0 spec review — `design/SPEC_tui_edit_chunk5.md` — ROUND 2 (post-fold verification)

**Reviewer role:** independent, adversarial architect. Did NOT author the spec or the folds.
**Baseline:** `feat/tui-edit-chunk5` @ `f31c1d6` (= current HEAD).
**Scope:** round-1 was already 0C/0I; this round re-verifies the M1/N1/N2 + cosmetic folds and hunts for NEW drift the folds could have introduced (esp. the `(Vec<AllocLot>, LotMethod)` signature change and the config-read-once restructure).

## Verdict: **0 Critical / 0 Important / 0 Minor / 0 Nit — R0-GREEN.** Fold is clean; no new drift.

---

## Fold-by-fold verification

### [M1] — helper returns the method it used; single config read — CLEAN, compiles, behavior-preserving.
New D3 body (spec:122-139):
```rust
let cfg = self.config()?;                 // ONE read
let pre2025_method = cfg.pre2025_method;  // returned
let proj = cfg.to_projection();           // residue computed under the SAME read
...
Ok((lots, pre2025_method))
```
- **Compiles:** `CliConfig` is `#[derive(..., Copy, ...)]` (`config.rs:10`) and `to_projection(self)` takes `self` by value (`config.rs:30`); because `CliConfig: Copy`, reading `cfg.pre2025_method` then calling `cfg.to_projection()` is not a move-after-use error. `LotMethod` is `Copy` (`project/mod.rs:24-25`), so both the field read and the tuple return are copies. No borrow/lifetime issue.
- **Structural consistency achieved:** the returned `pre2025_method` and the `proj` the residue is projected under now come from the SAME `self.config()?` — a divergent second read is eliminated by construction, which is exactly what M1 asked for.
- **CLI refactor stays behavior-preserving:** the command records the RETURNED `pre2025_method`. Old code recorded `cfg.pre2025_method` where `cfg = session.config()?.to_projection()`; `ProjectionConfig.pre2025_method == CliConfig.pre2025_method` (`to_projection` copies it, `config.rs:30-36`), and both reads hit the same config table in the same locked session → the appended payload's `pre2025_method` is byte-identical to before. The gate still does its own `session.config()?` read (attested check + error-message method tag only — NOT the recorded method), so the gate wording is unchanged. Empty-check now runs on the helper's returned `lots` — same predicate as before. `reconcile.rs` tests `570`/`733`/`828` still pin it; the strengthened `safe_harbor_residue_matches_command_lots` KAT (lots AND method) closes the coverage gap round-1 flagged.
- **TUI opener (D1 step 4/6):** now destructures `(lots, pre2025_method)` and records the RETURNED method at step 6 — the recorded tag is structurally the residue's method. Step 3's separate `config()?` read is used only for `pre2025_method_attested` (the message uses a literal `<m>` placeholder, so it needs no method value). Clean.
- **"Modeled EXACTLY on `optimize_proposal`":** the tuple return differs in arity from `optimize_proposal`'s single value, but the load-bearing pattern (held-conn, read-only, appends/persists nothing) is preserved. Benign, idiomatic — not drift.

### [N1] — KAT-G1 token list corrected — ACCURATE.
D3 (spec:113-115) now lists the gated persist-only tokens as `conn(`/`save(`/`append_`/`restore(`/`tax_profile::set`/`donation_details::set`/`optimize_attest::set` and explicitly notes `load_all`/`project` are NOT gated — matches `persist.rs:1224-1232` exactly. The "call site is clean" conclusion is unchanged and correct (`session.safe_harbor_residue()` contains none of them; the helper body lives in btctax-cli, outside the scanned crate).

### [N2] — arm-2 `event:None` exception justified — CLEAR.
D6 (spec:195-199) now annotates the `event:None` multiple-effective read as a deliberate exception to the new_id-only [R0-M10] discipline (no single owning id; defensive; stale-free; normally unreachable given the step-5 guard). Consistent with `resolve.rs:961-965`; preempts a future reviewer re-flagging it.

### Cosmetic — `is_revocable_payload` citation — FIXED.
Grounding (spec:46) now cites `form.rs:841` (verified exact).

---

## New-drift sweep (folds did not disturb anything else)
- D5 `persist_safe_harbor_allocate` still takes `pre2025_method: LotMethod` and records it verbatim — matches the D1/D3 thread; single-append template unchanged (`persist.rs:158-173`).
- D2 flow/modal structs still carry `pre2025_method: LotMethod` — the value now originates from the helper's return; types unchanged.
- No citation touched by the folds regressed: `reconcile.rs` 26-34/240-241/246-248/250-323/264-279/281-310, `resolve.rs` 859-866/872-882/924-934/946-955/961-965, `event.rs` 145-174, `conventions.rs` 17/19, `session.rs` 158, `config.rs` 10/30/123, `main.rs` 415/2558/4657/4712/4912/5259/5488 all still resolve.

**R0-GREEN — clear to proceed to implementation (Task 1 first: the helper + CLI refactor).**
