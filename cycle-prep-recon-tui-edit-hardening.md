# cycle-prep recon — 2026-07-02 — tui-edit chunk-3 FOLLOWUPS

**Origin SHA at recon time:** `7ba67a1` (origin/main — BEHIND; chunk-3 merge not pushed)
**Local branch:** `main` @ `45b9332` (just absorbed the chunk-3 `--no-ff` merge — the source of truth for this recon)
**Sync state:** 9 ahead / 0 behind origin/main
**Untracked:** none (working tree clean)

Items verified: the 9 FOLLOWUPS under `FOLLOWUPS.md` "✅ Mutating-TUI chunk 3 … SHIPPED (2026-07-02)".
Expectation: near-zero content drift (written + reviewer-verified today); the only systematic issue is a
path-prefix omission on engine-file citations.

## Decisions (2026-07-02, user-confirmed)

- **Cycle name:** `tui-edit-hardening` (a distinct hardening cycle — NOT a feature-chunk number; the feature
  roadmap remains 4 = import, 5 = safe-harbor-allocate/optimize).
- **Scope:** the **7 actionable items** — Group A (#1 SelfTransfer inclusion, #2 pre-2025 Universal-pool
  wallet filter, #3 shortfall guard) + Group B (#6 FIELD_CAP bump, #7 void-list effective-allocation
  pre-filter, #8 quit-first status fold, #9 session-dirty latch). **Excluded:** #4 (= planned chunk 5, a
  feature) and #5 (informational assurance note — no build).
- **Next phase (not yet started):** Brainstorm → Spec → mandatory R0 architect review to 0C/0I → Plan → R0 →
  phased TDD implementation → whole-diff review → ship. Awaiting user go to begin the brainstorm.

---

## Roadmap / numbering finding (the user's explicit question)

**There is NO `chunk 6` or `chunk 7` anywhere in the repo** (`git grep -niE 'chunk[ -_]?6|chunk[ -_]?7'` →
empty). The mutating-TUI program's actual roadmap, from the specs + FOLLOWUPS:

| # | Chunk | Status |
|---|---|---|
| 1 | tax-profile set/edit | shipped |
| 2a | classify-inbound + reclassify-outflow | shipped |
| 2b | reclassify-income + set-fmv + void | shipped |
| 3 | select-lots + set-donation-details + safe-harbor-attest | **shipped today (`45b9332`)** |
| 4 | import-level decisions (link-transfer, classify-raw, accept/reject-conflict, optimize-accept) | planned, not started |
| 5 | safe-harbor-**allocate** (the creation side) / optimize | planned, not started |

**Reconciling the user's belief:** "6 chunks" is correct *if you count 2a and 2b as two* → {1, 2a, 2b, 3, 4, 5}
= 6 chunk-artifacts. But the highest chunk **number** is **5**. "We are on 4 now" ≈ right: chunk 3 just
shipped, chunk 4 is the next feature up (un-started). **"Call it chunk 7" does not fit:** it skips the
non-existent chunk 6 and overshoots the numeric max (5). The next *free* number is **6**.

**Recommendation (naming is the user's call — flagged):** this cleanup is *cross-cutting hardening*, orthogonal
to the two remaining **feature** chunks (4 = import, 5 = safe-harbor-allocate). Forcing it into the feature-chunk
number line is misleading. Prefer a **distinct name** — e.g. `tui-edit-hardening` / "chunk-3 follow-up cleanup" —
over a feature-chunk number. If a mutating-TUI number is desired anyway, use **6** (next free), not 7.

---

## Per-item verification

Engine files `resolve.rs` / `fold.rs` / `pools.rs` live under **`crates/btctax-core/src/project/`** (subdir);
the FOLLOWUPS/reviews cite them by bare filename. Line numbers are content-accurate — only the `project/`
prefix is missing. Flagged once here rather than per line.

### #1 — SelfTransfer select-lots under-inclusion
- **WHAT:** linked TransferOut→`Op::SelfTransfer` events are method-honoring but absent from the `s` list.
- **Citations:**
  - `honoring_principal` returns `Some` for `Op::SelfTransfer` — **ACCURATE** (`project/resolve.rs:1008` fn, `:1013` `Op::SelfTransfer { sat, .. } => Some(*sat)`).
  - Absent from `state.disposals`/`state.removals` — **ACCURATE** (opener builds only from those two; SelfTransfer principal not exposed there).
  - Fix "scan `snap.events` for TransferOut with a non-voided TransferLink" — plausible design note; TransferLink existence to be confirmed at spec time.
- **Action:** real feature (new list source + a `select-lots`-through-SelfTransfer KAT). Cite `project/resolve.rs` @ `45b9332`.

### #2 — Lot-display-at-date + [ENG-m1] pre-2025 Universal-pool wallet filter
- **WHAT:** the candidate-lot filter is per-wallet, but pre-`TRANSITION_DATE` disposals consume from the un-partitioned Universal pool.
- **Citations:**
  - candidate filter `l.wallet == w` — **ACCURATE** (`crates/btctax-tui-edit/src/main.rs:2680` `.filter(|l| wallet_ref.is_some_and(|w| &l.wallet == w))`).
  - `PoolKey::Universal` is un-partitioned by wallet — **ACCURATE** (`project/pools.rs:11` `enum PoolKey { Universal, Wallet(WalletId) }`) — **note: DRIFTED** from the review's cited `pools.rs:15-21` to `:11`.
  - `TRANSITION_DATE` — **ACCURATE** (`crates/btctax-core/src/conventions.rs:17` `pub const TRANSITION_DATE: TaxDate = date!(2025-01-01)`).
- **Action:** small fix — drop the wallet filter when `item.date < TRANSITION_DATE` + KAT. Cite `project/pools.rs:11`, `conventions.rs:17`.

### #3 — [ENG-m2] shortfall-disposal principal target
- **WHAT:** for `UncoveredDisposal`, `Σ legs.sat < op.sat`, so the in-TUI conservation target undershoots the engine's.
- **Citations:** `validate_select_lots` — **ACCURATE** (`form.rs:932`); `derive_select_lots_status` LotSelectionInvalid Arm 2 — **ACCURATE** (`main.rs:3408-3410`); engine `honoring_principal` conservation — **ACCURATE** (`project/resolve.rs`, ~811-820).
- **Action:** one-line guard + test; low priority (degenerate; already surfaced). 

### #4 — Safe-harbor-**allocate** TUI flow  ⟵ EXCLUDE from the cleanup cycle
- **WHAT:** the CREATION side of the allocation.
- **Finding:** this **IS the planned chunk 5**, a full feature — not a cleanup item. Do NOT fold into the hardening cycle; it belongs to its own feature chunk.

### #5 — WB-I4(a) carryforward  ⟵ informational, no action
- **WHAT:** confirms the 2b raw-vs-effective under-inclusion does NOT affect chunk 3. A negative/assurance note; nothing to build.

### #6 — FIELD_CAP=64 CLI-parity limit
- **Citations:** **ACCURATE** — `pub const FIELD_CAP: usize = 64;` (`form.rs:18`); free-text fields truncate at it.
- **Action:** small — larger cap for designated free-text fields (addresses, `appraiser_qualifications`).

### #7 — Void-list pre-filter for effective allocations [R0-I6]
- **WHAT:** the 2b void flow still LISTS an effective (attested) allocation; a confirmed void is a permanently-damaging §7.4 no-op.
- **Citations:** `is_revocable_payload` includes `SafeHarborAllocation` — **ACCURATE** (`form.rs:824` fn, `:836` `| EventPayload::SafeHarborAllocation(_)`); trap pinned by KAT-E2E-ATTEST-VOID (chunk-3 tests) — **ACCURATE**.
- **Action:** medium — pre-filter effective allocations out of the void list (effectiveness derivable from blockers) + KAT. Genuine trap-closer.

### #8 — [SAFE-M2] pre-existing 2a/2b void-remedy statuses omit "quit the editor first"
- **Citations:** `derive_classify_inbound_status` (`main.rs:1991`), `derive_reclassify_income_status` (`:2287`), `derive_set_fmv_status` (`:2308`) — all **ACCURATE**. Confirmed wording: `"… clear with Void flow (press 'v') or CLI: btctax reconcile void {}"` — names the in-editor `v` remedy first, omits the quit-first clause. Present at `main` pre-chunk-3.
- **Action:** small — apply the R0-C1 quit-first fold to the 3 strings + update their KAT asserts.

### #9 — Session-dirty latch generalization
- **Citations:** `attest_save_failed` — **ACCURATE** (`editor.rs`, the single-purpose C1 latch).
- **Action:** medium/architectural — generalize into a session-dirty latch covering all failed saves (2a/2b keep-form-open flows too).

---

## Cross-cutting observations

1. **Path-prefix omission (systematic, benign):** engine citations use bare `resolve.rs`/`fold.rs`/`pools.rs`; actual home is `crates/btctax-core/src/project/`. Qualify in any brainstorm spec.
2. **One DRIFTED line number:** `PoolKey` enum is at `project/pools.rs:11`, not the `:15-21` the round-1 review cited (content identical).
3. **Two non-cleanup items mixed into the list:** **#4 = the planned chunk 5** (a feature, exclude) and **#5 = informational** (no action). The actionable cleanup set is **7 items: #1, #2, #3, #6, #7, #8, #9.**
4. **No CLI-surface / clap-flag changes** in the actionable set → **no `schema_mirror` lockstep, no `docs/manual/` CLI-reference mirror** required. #8 changes user-facing *status strings* (not a documented CLI surface); #6 changes an input cap. Confirm no manual/help text references these at spec time.
5. **Sync:** local `main` is 9 ahead of origin/main (chunk-3 not pushed); this recon is against local `main` @ `45b9332` per instruction.

---

## Recommended brainstorm-session scope

**One hardening cycle, 7 actionable items, sub-grouped; exclude #4 (=chunk 5) and #5 (informational).**

- **Group A — select-lots correctness (cohesive; do together):** #1 SelfTransfer inclusion (medium; new list source + KAT), #2 pre-2025 Universal-pool wallet filter (small), #3 shortfall guard (small). ~1 spec; touches `open_select_lots_flow` + the candidate-lot filter + `validate_select_lots`.
- **Group B — safety/UX hardening:** #7 void-list effective-allocation pre-filter (medium; genuine trap-closer — highest safety value), #8 quit-first status fold on the 3 pre-existing 2a/2b statuses (small), #9 session-dirty latch generalization (medium/architectural — sequence AFTER #8 since both touch the failed-save/status surface), #6 FIELD_CAP bump (small, standalone).
- **Rough sizing:** Group A ≈ 150–250 LOC + tests; Group B ≈ 200–350 LOC + tests. Total well within one cycle.
- **SemVer:** all additive / internal → **MINOR (pre-1.0)**; no breaking changes.
- **Locksteps:** none mandatory (no clap flag-name changes → no GUI `schema_mirror`; no CLI-reference surface → no manual mirror). Verify at spec time that no `docs/manual/` help text quotes the changed status strings (#8) or the 64-char cap (#6).
- **Ordering / dependencies:** #8 → #9 (latch generalization builds on the status-honesty fold). #1/#2/#3 independent of Group B. #7 independent but highest-priority (closes a permanently-damaging trap). No dependency on chunks 4/5.
- **Naming:** recommend a distinct `tui-edit-hardening` cycle over a feature-chunk number (see roadmap finding). If numbered anyway, **6**, not 7.

**Next gate:** this is recon only. A brainstorm spec / implementation plan for these items MUST pass the mandatory R0 architect review to 0 Critical / 0 Important before any implementation.
