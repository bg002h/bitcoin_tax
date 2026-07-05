# R0 — SPEC_sort_views.md — round 3 (independent architect)

**Artifact:** `design/SPEC_sort_views.md` (round-2 folded).
**Baseline:** branch `feat/sort-views` @ `6b60a0f`; `main` @ `1837917`. Repo `/scratch/code/bitcoin_tax`.
**Reviewer role:** independent architect (author ≠ reviewer). Read-only; no implementation.
**Bar:** 0 Critical / 0 Important. Prior rounds: `reviews/R0-spec-sort-views-round-{1,2}.md`
(round 1 0C/4I/4M/2N; round 2 0C/1I/3N).
**Mandate this round:** confirm ONLY the round-2 folds (I5 blocker + N-1 + N-3) against source, plus a
whole-spec final sanity / Plan-implementability pass. Rounds 1–2 already verified every prior finding resolved.

## Verdict: **0 Critical / 0 Important / 0 Minor / 1 Nit — R0-GREEN (cleared to implement).**

The one round-2 blocker (I5, the 23-test rebind migration) is **folded correctly and verified exhaustively
against source**: the count (15 `s` + 8 `l` = 23), every cited line, the "top-level opener" classification, and
the "flow handlers bind neither key" justification all check out — and the enumeration is **complete** (there
are no other top-level `s`/`l` test presses, and no `S`/`L` collision). N-1 (`IncomeRecord`) and N-3 (`l`
top-level, freed by the rebind) are both folded and correct. No source changed between the round-2 baseline
(`67b62de`) and now (`6b60a0f`) — `git diff --stat 67b62de 6b60a0f` shows only the spec + the round-2 review
committed — so every citation verified in prior rounds remains current. The doc is internally consistent after
three edits, and the Plan (T1/T2/T3) is fully implementable with 0 open questions. The single Nit is a
non-blocking round-2 carryover (N-2) that I re-verified does not break the suite.

---

## Round-2 fold verification ledger

### I5 (round-2 blocker) — MIGRATE the 23 browse-level `s`/`l`→`S`/`L` opener tests → **RESOLVED ✔ (enumeration complete)**

SPEC now owns the migration in three places — the keymap note (line 24: "**[R0-I5] The rebind ALSO breaks 23
browse-level tests…**"), a dedicated KAT bullet (lines 95-99), and Plan **T2** (line 116, "**[I5] migrate the 23
browse `s`/`l`→`S`/`L` opener tests** (mechanical find-and-replace)"). Every factual claim in that fold is
verified against `crates/btctax-tui-edit/src/main.rs`:

- **Top-level bindings** are exactly where the spec says: browse handler `KeyCode::Char('s') =>
  open_select_lots_flow` at **main.rs:421**, `KeyCode::Char('l') => open_link_transfer_flow` at **main.rs:423**
  (both inside the Browse `match key.code` block, lines 397-441). ✔
- **The 15 `s` presses** — an exhaustive `grep "Char('s')"` returns 16 hits: the :421 binding + exactly 15 test
  presses `handle_key(&mut app, press(KeyCode::Char('s')))` at main.rs **14735, 14962, 15212, 15305, 15339,
  15424, 15479, 17516, 17613, 17767, 17907, 18030, 18095, 18216, 18402**. Exact match to the spec's
  "~14735-18402" range and to the round-2 enumeration. ✔
- **The 8 `l` presses** — an exhaustive `grep "Char('l')"` returns 11 hits: the :423 binding + 2 **in-flow**
  handlers (main.rs **7138** and **8271**, both `KeyCode::Left | KeyCode::Right | KeyCode::Char('h') |
  KeyCode::Char('l')` cursor arms in the resolve-conflict / bulk-resolve contexts — NOT browse) + exactly 8 test
  presses at main.rs **18328, 18449, 18521, 18632, 18706, 18805, 18879, 19478**. Exact match to the spec's
  "~18328-19478". ✔  (15 + 8 = 23, consistent with the header line 4, line 24, and line 95.)
- **Flow handlers bind neither `s` nor `l` internally** — verified by reading the bodies:
  `handle_select_lots_flow_key` at **main.rs:3197** (→ `handle_sl_list_key` / `handle_sl_lots_form_key`) and
  `handle_link_transfer_flow_key` at **main.rs:4247** (→ `handle_lt_out_list_key` / `handle_lt_target_pick_key`)
  use only j/k/g/G/Enter/Esc/Tab/q + a `_ => {}` catch-all. The whole-file grep corroborates: no `Char('s')` or
  `Char('l')` exists anywhere in the 3197-4500 handler range. So every `press(Char('s'/'l'))` in a test reaches
  the **top-level** browse arm — these are genuine openers, exactly as the spec claims. ✔
- **All 23 are genuinely browse-level** (so a mechanical `s→S` / `l→L` replace preserves intent) — spot-checked
  the three `is_none()` cases the round-2 review flagged and two `l` cases:
  - main.rs:15305 (`app.status = None; press('s'); assert select_lots_flow.is_none()` — pre-filter, browse),
    17613 (`open_app(...) ; press('s'); assert is_none()` — voided link → empty list, browse), 18216
    (browse press, under-covered pre-filter → is_none). Each is set up at Browse; after the rebind `S` will
    carry the same intent. ✔
  - main.rs:18521 (`press('l'); assert link_transfer_flow.is_some(), "C2-LT: flow opens on 'l'"`) and 19478
    (`open_app(...); press('l')` then renders the "Link Transfer" overlay) — both browse-level openers. ✔
- **No OTHER top-level `s`/`l` usages are missed.** The only alternative key-input path in the suite is the
  `type_str` helper (main.rs:8962, `for c in s.chars() { handle_key(app, press(KeyCode::Char(c))) }`) and the
  inline `for c in "…".chars()` loops. Every such call feeds a string INTO AN OPEN FLOW/FORM buffer or the
  UNLOCK screen (passphrases, amounts, names like "Community Foundation", "self:cold", "ATTEST", …) — i.e. after
  a flow is open, `handle_key` dispatches to the flow's `Char(c)` form handler, never the browse `Char('s')`/
  `Char('l')` arm. So the 's'/'l' characters inside those strings are form data, correctly excluded from the 23.
  The accounting is therefore **complete**, not merely representative. ✔
- **No `S`/`L` collision.** `grep "Char('S')"` and `grep "Char('L')"` over the editor `main.rs` return **NONE**
  — no existing test presses `S`/`L` expecting some other behavior, and `S`/`L` are unbound in the browse
  handler (lines 397-441 bind `A/B/I/C/V/O/P/G` uppercase but not `S`/`L`). The rebind targets are free, and the
  migrated tests' `press(KeyCode::Char('S'/'L'))` (built with `KeyModifiers::NONE`, matched purely on
  `key.code`) will hit the new arms cleanly. ✔

The "mechanical find-and-replace in T2" is now an accurate, fully-enumerated instruction — the understated
blast-radius that blocked round 2 is closed.

### N-1 — Income working-set type name → **RESOLVED ✔**
Zero stray `IncomeRecognized` remain in the spec (grep: none). Both former occurrences now read `IncomeRecord`
— line 38 ("`IncomeRecord` has no wallet field") and line 69 ("Income `Vec<&IncomeRecord>`"). Confirmed the real
type is `struct IncomeRecord` at **state.rs:211-218** with fields `event / recognized_at / sat / usd_fmv / kind
/ business` — **no wallet field** (substance of I3/N-1 holds) — and the `LedgerState` collection is
`pub income_recognized: Vec<IncomeRecord>` at **state.rs:255**. `Vec<&IncomeRecord>` now compiles as written. ✔

### N-3 — `h`/`l` flow-vs-top-level phrasing → **RESOLVED ✔**
SPEC line 22-24 now reads: "`h` appears only inside flow contexts (main.rs:7138/8271); `l` IS bound top-level
(main.rs:423) and is freed by this rebind [R0-N-3]." Verified against source: `grep "Char('h')"` returns exactly
main.rs **7138** and **8271** (both in-flow) — so `h` is unbound at top level and free for cursor-left; `l` is
bound top-level at **423** (the link-transfer binding being rebound to `L`) and additionally in-flow at
7138/8271 (those in-flow arms are untouched by the rebind, per line 80-81). The reworded sentence is precise and
the conflict-free conclusion holds. ✔

---

## Whole-spec sanity + Plan implementability

- **Round-2 fold diff is clean and surgical.** `git show 6b60a0f -- design/SPEC_sort_views.md` touches only the
  status header, the I5 keymap note + KAT bullet + T2 line, the two `IncomeRecognized→IncomeRecord` renames, and
  the N-3 rewording. No unrelated edits; no new contradiction introduced.
- **No residual contradiction across the 3-times-edited doc.** The "23 = 15 + 8" figure is consistent in the
  header (line 4), keymap (line 24), and KATs (lines 95-99). The finalized key map (`s` sort, `[`/`]` year,
  editor `S`/`L` rebind) is stated identically in the keymap table, the Architecture "Keys" bullet (lines
  79-81), Scope (lines 102-104), and the Plan (T1/T2). Column lists (Holdings/Disposals/Income) match the render
  code and the default-sort/semantics sections.
- **Plan is fully implementable with 0 open questions:**
  - **T1 (viewer)** — the load-bearing citations are current: viewer key handler / year keys `lib.rs:215/219`
    (Left `selected_year -= 1`, Right `selected_year += 1`, both with `reset_selections`), and the existing
    arrow=year test it must update, `left_right_changes_selected_year` at **lib.rs:776**. ✔
  - **T2 (editor)** — editor year keys `main.rs:407/411` (Left/Right), the `s`/`l` rebind sources 421/423, the
    render call sites, the sole editor arrow=year test `left_right_on_browse_changes_selected_year`
    (`#[test]` at main.rs:9493 / fn at 9494 — the spec's "main.rs:9493" points at the attribute; the grep
    confirms it is the *only* such editor test), and the now-complete 23-test migration. ✔
  - **T3 (docs)** — hand-authored man pages + footer/overlay + README; round-2 verified, source unchanged. ✔

---

## Nit (non-blocking — does not affect the 0C/0I bar)

### N-2 (carryover) — the `docs.rs` man-page sync-test coupling is still unmentioned, but verified not to break
Round-2's N-2 asked for a one-line note that the T3 hand-edits keep the `manpages_have_required_sections` sync
test green. The round-2 fold did not add it (grep of the spec: no mention). I re-verified the test is unaffected:
`xtask/src/docs.rs:410-415` asserts `btctax-tui-edit.1` contains `\n.B ?`, `\n.B V`, `\n.B O`; the planned
hand-edits (year `←/→`→`[`/`]`, `s`→`S`, `l`→`L`, add `S`/`L`/sort rows) touch none of the `?`/`V`/`O` entries,
so the sync test stays green. Purely informational — worth a one-line note in §I4/T3 when the implementer adds
the `S`/`L`/sort man-page rows, but it blocks nothing.

*(Trivial, sub-nit: the T2 continuation line 116 `(`main.rs:9493`) + **[I5] migrate…** ` starts at column 0
rather than indented under the bullet; markdown lazy-continuation still renders it as part of the T2 item, so it
is cosmetic only — flag purely for completeness.)*

---

## Confirmed sound (spot-checked this round)
- `Char('[')` / `Char(']')` return **NONE** in the editor `main.rs` — the year-move keys are free for binding. ✔
- The browse handler (main.rs:397-441) binds `A/B/I/C/V/O/P/G` uppercase but **not** `S`/`L` — rebind targets
  are conflict-free. ✔
- The two in-flow `l` arms (main.rs:7138/8271) are resolve-conflict / bulk-resolve cursor handlers, a separate
  match context from Browse — untouched by the top-level rebind, consistent with SPEC lines 80-81. ✔
- No source drift since the round-2 baseline (`git diff --stat 67b62de 6b60a0f` = spec + round-2 review only). ✔

## Re-review note
0 Critical / 0 Important. The one round-2 blocker (I5) and both round-2 Nits that were folded (N-1, N-3) are
verified resolved against current source; the remaining Nit (N-2) is a non-blocking documentation note the
round-2 reviewer already classified as non-breaking. Per the workflow's "re-review after every fold, including
the last," this round confirms the last fold. **R0-GREEN — cleared to implement.**
